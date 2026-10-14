use super::special::tile_period;

use super::*;

#[test]
fn scripted_emission_preserves_fritz_warp_alpha_glow_base_and_inclusive_thresholds() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 16,
        special_parms: [[0.5, 1., 0., 0.], [0., 0.5, 0., 0.], [0.; 4]],
        inputs: vec![
            Input {
                width: 2,
                height: 1,
                sampler: 20,
                pixels: vec![[0.2, 0.4, 0.6, 0.2], [0.8, 0.6, 0.4, 0.8]],
            },
            input([0.5, 0.5, 1., 1.]),
            Input {
                width: 2,
                height: 1,
                sampler: 20,
                pixels: vec![[0.2, 0.3, 0.4, 0.7], [0.8, 0.6, 0.4, 0.1]],
            },
            Input {
                width: 2,
                height: 1,
                sampler: 20,
                pixels: vec![[0.9, 0.8, 0.7, 0.25], [0.1, 0.2, 0.3, 0.75]],
            },
            input([1., 0.5, 0., 0.]),
            input([0.8, 0.6, 0.4, 0.5]),
        ],
        base_specular: true,
        geometric_normal: false,
        second_additive: false,
        second_normal: 0,
        second_reveal: false,
        second_specular: false,
        second_parms: [0., 0.05, 0.5, 1.],
        third_multiply: false,
        layer_cutout: false,
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: false,
        parms: [1., 1., 2., 4.],
    };
    recipe.inputs[5].sampler = 2;
    let sample = [
        0.25, 0.5, 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.,
    ];
    let base = gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
    assert_eq!(base[0][3], 0.2);
    assert_eq!(base[3][3], 0.7);
    let emission = recipe.emission_pixel([0.25, 0.5], base[2][3]);
    for (actual, expected) in emission.into_iter().zip([0.050625, 0.04, 0.030625, 1.]) {
        assert!((actual - expected).abs() < 0.00001);
    }
    recipe.special_parms[1][0] = 0.5;
    let actual = gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
    assert_eq!(actual[0][3], 0.8);
    assert_eq!(actual[3][3], 0.7);
    let expected = recipe.sample_special([0.25, 0.5]).unwrap();
    for (a, b) in actual
        .into_iter()
        .flatten()
        .zip(expected.into_iter().flatten())
    {
        assert!((a - b).abs() < 0.00003);
    }
    let luma = 0.638_f32 * 0.638 * 0.5;
    let glow: [f32; 3] = std::array::from_fn(|axis| {
        let base = [0.2, 0.4, 0.6][axis];
        let target = luma + 0.0625 * ([0.8, 0.6, 0.4][axis] - luma);
        base + 0.5 * (target - base)
    });
    let emission = recipe.emission_pixel([0.25, 0.5], actual[2][3]);
    for axis in 0..3 {
        assert!((emission[axis] - (glow[axis] * 0.75).powi(2)).abs() < 0.00001);
    }
    recipe.special_parms[0][1] = 0.;
    assert_eq!(
        gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0][0][3],
        1.
    );
    recipe.special = 32;
    recipe.parms = [0., 0., 0., 2.];
    recipe.second_parms = [1., 1., 0.7, 1.];
    for (state, weight) in [
        (f32::from_bits(0.7_f32.to_bits() - 1), 0.),
        (0.7, 4.),
        (f32::from_bits(0.7_f32.to_bits() + 1), 4.),
    ] {
        recipe.special_parms[1][2] = state;
        let actual = gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
        assert_eq!(actual[2][3], weight);
        assert_eq!(actual[0][3], 0.2);
    }
    recipe.special = 64;
    recipe.parms = [1., 1., 0., 3.];
    recipe.special_parms[1] = [2., 0.5, 1., 0.25];
    recipe.inputs[3] = input([0.8, 0.6, 0.4, 0.5]);
    let actual = gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
    assert_eq!(actual[2][3], 9.);
    for (actual, expected) in recipe
        .emission_pixel([0.25, 0.5], actual[2][3])
        .into_iter()
        .zip([0.36, 0.01265625, 0.0225, 1.])
    {
        assert!((actual - expected).abs() < 0.00001);
    }
}

#[test]
fn phong_emission_push_and_flicker_controls_preserve_source_amplitude_and_hdr_response() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 4,
        special_parms: [[0.; 4]; 3],
        inputs: vec![
            input([0.2, 0.4, 0.6, 0.25]),
            input([0.5, 0.5, 1., 1.]),
            input([0.4, 0.5, 0.6, 0.8]),
            input([0.8, 0.6, 0.4, 0.25]),
        ],
        base_specular: true,
        geometric_normal: false,
        second_additive: false,
        second_normal: 0,
        second_reveal: false,
        second_specular: false,
        second_parms: [1., 1., 0., 0.],
        third_multiply: false,
        layer_cutout: false,
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: false,
        parms: [0.5, 1.25, 2., 3.],
    };
    let sample = [
        0.375, 0.625, 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.,
    ];
    let actual = gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
    assert!((actual[2][3] - 144.).abs() < 0.00001);
    for (actual, expected) in actual[3].into_iter().zip([0.04, 0.0625, 0.09, 1.]) {
        assert!((actual - expected).abs() < 0.00001);
    }
    assert_eq!(actual[0][3], 1.);
    recipe.special_parms[0][0] = 1.;
    assert_eq!(
        gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0][0][3],
        0.25
    );
    let image = mip_chain(1, 1, vec![[1.5, 0.2, 0.3, 1.24]], "native-specular").unwrap();
    assert_eq!(image.format, 19);
    let pixel = texture::pixels(&image, 0).unwrap()[0];
    assert_eq!(pixel[0], 1.5);
    assert!((pixel[3] - 1.24).abs() < 0.001);
    texture::validate(&texture::encode(&image, false, "reflectance").unwrap()).unwrap();
    recipe.special = 8;
    recipe.parms = [3., 0.1, 0.2, 0.8];
    recipe.second_parms = [0.3, 0., 0., 0.];
    recipe.inputs[3] = input([0.25; 4]);
    let actual = gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
    assert_eq!(actual[2][3], 9.);
    assert_eq!(actual[3], [0.04, 0.04, 0.04, 0.25]);
    let expected = recipe.sample_special([sample[0], sample[1]]).unwrap();
    for (a, b) in actual
        .into_iter()
        .flatten()
        .zip(expected.into_iter().flatten())
    {
        assert!((a - b).abs() < 0.00003);
    }
    for (actual, expected) in recipe
        .emission_pixel([0.375, 0.625], 9.)
        .into_iter()
        .zip([0.9216, 1.44, 2.0736, 1.])
    {
        assert!((actual - expected).abs() < 0.00001);
    }
}

#[test]
fn special_surface_gpu_preserves_macro_mask_response_channels_and_bound_water_states() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 1,
        special_parms: [[0.; 4]; 3],
        inputs: vec![
            input([0.1, 0.5, 0.9, 0.]),
            input([0.6, 0.2, 1., 1.]),
            input([0.8, 0.3, 1., 1.]),
            input([0.2, 0.9, 1., 1.]),
            input([0.2, 0.4, 0.6, 0.]),
            input([0.8, 0.2, 0.4, 1.]),
            input([0.2, 0.4, 0.5, 0.6]),
            input([0.8, 0.8, 0.3, 0.2]),
        ],
        base_specular: true,
        geometric_normal: false,
        second_additive: false,
        second_normal: 0,
        second_reveal: false,
        second_specular: false,
        second_parms: [2., 3., -0.25, 4.],
        third_multiply: false,
        layer_cutout: false,
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: false,
        parms: [3., 3., 2.5, 2.5],
    };
    assert_eq!(tile_period(&recipe).unwrap(), [2, 2]);
    let sample = [
        0.375, 0.625, 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.,
    ];
    let actual = gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
    for (a, b) in actual.into_iter().flatten().zip(
        recipe
            .sample_special([sample[0], sample[1]])
            .unwrap()
            .into_iter()
            .flatten(),
    ) {
        assert!((a - b).abs() < 0.00003);
    }
    for (actual, expected) in actual[3].into_iter().zip([0.25, 0.2025, 0.25, 0.35]) {
        assert!((actual - expected).abs() < 0.00001);
    }
    recipe.parms[0] = f32::from_bits(0x3f800001);
    assert!(tile_period(&recipe).is_err());
    recipe.special = 2;
    recipe.parms = [0.125, 0.5, 0.6, 1.2];
    recipe.second_parms = [6., 8., 8., 2.];
    recipe.special_parms = [[4., 4., 4., 2.5], [0., 0., 0.5, 0.], [0., 0., 0., 1.25]];
    recipe.inputs = vec![
        input([0.5, 0., 0., 0.]),
        input([0.6, 0.4, 1., 1.]),
        input([1.; 4]),
        input([0.8, 0.8, 1., 1.]),
        input([0.4, 0.5, 0.6, 1.]),
        input([0.2, 0.3, 0.4, 0.25]),
    ];
    for positive in [false, true] {
        recipe.inputs[3].pixels[0] = if positive {
            [0.8, 0.8, 1., 1.]
        } else {
            [0.2, 0.2, 1., 1.]
        };
        for state in [0.2, 0.5, 0.8] {
            recipe.special_parms[1][2] = state;
            let actual = gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
            let expected = recipe.sample_special([sample[0], sample[1]]).unwrap();
            for (a, b) in actual
                .into_iter()
                .flatten()
                .zip(expected.into_iter().flatten())
            {
                assert!((a - b).abs() < 0.0001);
            }
            if state == 0.5 {
                assert!((actual[2][3] - 0.0625).abs() < 0.00001);
                assert!((actual[3][3] - if positive { 0.25 } else { 0.625 }).abs() < 0.00001);
            } else {
                assert_eq!(actual[2][3], 0.);
            }
        }
    }
}

#[test]
fn gpu_matches_existing_material_equations_and_sampler_modes() {
    let modes = [
        (
            false, false, false, false, false, false, false, false, false, false, 0, false, false,
            false, false,
        ),
        (
            true, false, false, false, false, false, false, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, false, false, true, false, false, false, false, false, 0, false, false,
            false, false,
        ),
        (
            true, false, false, false, true, false, false, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, false, false, true, true, false, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, false, false, true, true, true, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, false, false, false, false, true, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, false, false, true, false, true, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, true, false, false, false, false, false, false, false, false, 0, false, false,
            false, false,
        ),
        (
            true, true, false, false, false, false, false, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, true, false, false, false, false, true, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, true, false, false, false, true, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, false, true, false, false, true, false, false, false, 0, false, false,
            false, false,
        ),
        (
            true, false, false, true, false, false, true, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, false, false, false, false, false, true, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, false, false, true, true, false, true, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, false, false, false, true, true, false, false, false, 0, false, false,
            false, false,
        ),
        (
            true, false, false, false, true, true, true, false, false, false, 0, false, false,
            false, false,
        ),
        (
            false, false, false, true, false, false, true, true, true, false, 0, false, false,
            false, false,
        ),
        (
            true, false, true, false, false, false, true, false, false, false, 0, false, false,
            false, false,
        ),
        (
            true, false, false, false, false, false, true, false, false, true, 0, false, false,
            false, false,
        ),
        (
            true, false, false, false, true, true, true, false, false, true, 0, false, false,
            false, false,
        ),
        (
            true, false, false, true, false, false, true, false, false, false, 1, false, false,
            false, false,
        ),
        (
            true, false, false, false, false, false, true, false, false, false, 2, false, false,
            false, false,
        ),
        (
            true, false, false, false, true, true, true, false, false, false, 2, true, true, false,
            false,
        ),
        (
            true, false, false, false, false, false, false, true, false, false, 2, true, true,
            false, false,
        ),
        (
            true, false, false, false, true, true, false, true, false, false, 2, true, true, false,
            false,
        ),
        (
            true, false, false, true, false, false, true, false, false, false, 1, true, true,
            false, false,
        ),
        (
            true, false, false, false, true, true, true, false, false, false, 2, true, false,
            false, false,
        ),
        (
            true, false, false, false, false, false, true, false, false, false, 2, true, true,
            false, false,
        ),
        (
            true, false, false, true, false, false, true, false, false, false, 1, false, true,
            false, false,
        ),
        (
            true, false, false, false, true, true, true, false, false, true, 0, false, false, true,
            false,
        ),
        (
            false, false, false, false, false, false, true, false, false, false, 0, false, false,
            false, true,
        ),
    ];
    let mut maximum = 0_f32;
    for (
        mode,
        &(
            decal,
            multiply,
            additive,
            blend_color,
            reveal,
            layer_specular,
            base_alpha_response,
            base_specular,
            geometric_normal,
            second_additive,
            second_normal,
            second_reveal,
            second_specular,
            third_multiply,
            layer_cutout,
        ),
    ) in modes.iter().enumerate()
    {
        for sampler in [1, 20, 33, 65, 97, 42, 74, 106] {
            let inputs = (0..9)
                .map(|input| Input {
                    width: 3,
                    height: 5,
                    sampler,
                    pixels: (0..15)
                        .map(|index| {
                            std::array::from_fn(|channel| {
                                ((index * 7 + input * 11 + channel * 3) % 31) as f32 / 31.
                            })
                        })
                        .collect(),
                })
                .collect();
            let recipe = Recipe {
                alpha_reveal: false,
                special: 0,
                special_parms: [[0.; 4]; 3],
                third_multiply,
                layer_cutout,
                second_reveal,
                second_specular,
                second_parms: [0.3, if mode % 2 == 0 { 0. } else { 0.625 }, 0., 0.],
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
                parms: [0.5, 1.3, 0., 0.],
            };
            let samples: Vec<gpu::Sample> = (0..513)
                .map(|index| {
                    let u = (index % 23) as f32 / 7. - 1.5;
                    let v = (index % 17) as f32 / 6. - 0.5;
                    [
                        u,
                        v,
                        u + 0.31,
                        v - 0.27,
                        u - 0.61,
                        v + 0.43,
                        (index % 13) as f32 / 12.,
                        (index % 11) as f32 / 10.,
                        0.,
                        -1.,
                        1.,
                        0.,
                        0.6,
                        0.8,
                        -0.8,
                        0.6,
                        u + 0.73,
                        v - 0.81,
                        (index % 7) as f32 / 6.,
                        0.,
                    ]
                })
                .collect();
            let baker = gpu::Baker::new(&recipe).unwrap();
            let actual = baker.sample(&samples).unwrap();
            for (sample, actual) in samples.iter().zip(actual) {
                let expected = recipe
                    .sample_frames(
                        [sample[0], sample[1]],
                        [sample[2], sample[3]],
                        [sample[4], sample[5]],
                        sample[6],
                        sample[7],
                        sample[8..12].try_into().unwrap(),
                        sample[12..16].try_into().unwrap(),
                        sample[16..20].try_into().unwrap(),
                    )
                    .unwrap();
                for (channel, (actual, expected)) in actual
                    .into_iter()
                    .flatten()
                    .zip(expected.into_iter().flatten())
                    .enumerate()
                {
                    let error = (actual - expected).abs();
                    maximum = maximum.max(error);
                    assert!(
                        error <= 0.00003,
                        "mode {mode}, sampler {sampler}, channel {channel}: GPU {actual}, CPU {expected}, sample {sample:?}"
                    );
                }
            }
        }
    }
    println!(
        "GPU parity: {} recipes, 8 samplers, 513 samples each; maximum absolute channel error {maximum}",
        modes.len()
    );
    let input = |value| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![value],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.4, 0.7, 0.2, 0.3]),
            input([0.5; 4]),
            input([0.8, 0.2, 0.4, 0.7]),
            input([0.6; 4]),
            input([0.7; 4]),
        ],
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: true,
        layer_specular: true,
        base_alpha_response: true,
        parms: [0.5, 1.3, 0., 0.],
    };
    let baker = gpu::Baker::new(&recipe).unwrap();
    let sample = [
        0.25, 0.5, 0.25, 0.5, 0.25, 0.5, 0.6, 0., 1., 0., 0., 1., 1., 0., 0., 1., 0., 0., 0., 0.,
    ];
    let expected = baker.sample(&[sample]).unwrap()[0];
    let samples = vec![sample; 262_147];
    let begin = std::time::Instant::now();
    let actual = baker.sample(&samples).unwrap();
    let gpu_time = begin.elapsed();
    assert_eq!(actual.len(), samples.len());
    assert!(actual.into_iter().all(|pixel| pixel == expected));
    let begin = std::time::Instant::now();
    for sample in &samples {
        std::hint::black_box(
            recipe
                .sample(
                    [sample[0], sample[1]],
                    [sample[2], sample[3]],
                    [sample[4], sample[5]],
                    sample[6],
                    sample[7],
                    sample[8..12].try_into().unwrap(),
                )
                .unwrap(),
        );
    }
    println!(
        "262147 material samples including GPU transfer/readback: GPU {gpu_time:?}, serial CPU {:?}",
        begin.elapsed()
    );
    recipe.parms = [0.; 4];
    assert!(
        gpu::Baker::new(&recipe)
            .unwrap()
            .sample(&[sample])
            .unwrap_err()
            .to_string()
            .contains("thresholds")
    );
}

#[test]
fn authored_specular_preserves_uv0_and_decal_gloss_order() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: true,
        geometric_normal: false,
        inputs: vec![
            input([0.6, 0.5, 0.4, 0.1]),
            input([0.5, 0.5, 1., 1.]),
            input([0.2, 0.3, 0.4, 1.]),
            input([0.5, 0.5, 1., 1.]),
            input([0.4, 0.6, 0.8, 0.9]),
            input([0.8, 0.6, 0.4, 0.3]),
        ],
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: true,
        base_alpha_response: false,
        parms: [0.; 4],
    };
    assert_eq!(
        (0..6)
            .map(|index| recipe.input_uv(index))
            .collect::<Vec<_>>(),
        [0, 0, 1, 1, 1, 0]
    );
    let mixed = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.5, 0., [1., 0., 0., 1.])
        .unwrap();
    for (actual, expected) in mixed[3].into_iter().zip([0.36, 0.36, 0.36, 0.6]) {
        assert!((actual - expected).abs() < 0.000001);
    }
    recipe.inputs.pop();
    recipe.base_specular = false;
    recipe.base_alpha_response = true;
    recipe.decal = true;
    recipe.inputs.insert(4, input([0.2, 0.4, 0.6, 0.]));
    let no_decal = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.5, 0., [1., 0., 0., 1.])
        .unwrap();
    let decal = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.5, 1., [1., 0., 0., 1.])
        .unwrap();
    assert_eq!(&no_decal[3][..3], &decal[3][..3]);
    assert!((decal[3][3] - 0.5 * (0.2 * 0.3 + 0.4 * 0.6 + 0.6 * 0.1)).abs() < 0.000001);
    assert_eq!(no_decal[1], decal[1]);
    assert_eq!(
        (0..6)
            .map(|index| recipe.input_uv(index))
            .collect::<Vec<_>>(),
        [0, 0, 1, 1, 2, 1]
    );
    recipe.inputs = vec![
        input([0.2, 0.4, 0.6, 0.1]),
        input([0.8, 0.6, 0.4, 0.5]),
        input([0.4, 0.6, 0.8, 0.7]),
    ];
    recipe.geometric_normal = true;
    recipe.base_specular = true;
    recipe.layer_specular = false;
    recipe.blend_color = true;
    recipe.decal = false;
    let geometric = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 1., 1., [1., -1., -1., 1.])
        .unwrap();
    assert_eq!(geometric[1], [0., 0., 1., 1.]);
    assert!((geometric[3][3] - 0.7).abs() < 0.000001);
    assert_eq!(
        (0..3)
            .map(|index| recipe.input_uv(index))
            .collect::<Vec<_>>(),
        [0, 1, 0]
    );
}

#[test]
fn add_multiply_and_second_normal_follow_source_order() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.4, 0.5, 0.6, 0.8]),
            input([0.5, 0.5, 1., 1.]),
            input([0.8, 0.6, 0.4, 0.5]),
            input([0.2, 0.4, 0.6, 0.3]),
        ],
        decal: true,
        multiply: false,
        additive: true,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: true,
        parms: [0.; 4],
    };
    let first = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.5, 1., [1., 0., 0., 1.])
        .unwrap();
    for (actual, expected) in first[0][..3].iter().zip([0.0144, 0.0676, 0.1764]) {
        assert!((actual - expected).abs() < 0.000001);
    }
    assert!((first[2][3] - 0.288).abs() < 0.000001);
    recipe.additive = false;
    recipe.second_additive = true;
    recipe.inputs.insert(3, input([0.5, 0.5, 1., 1.]));
    let without = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.5, 0., [1., 0., 0., 1.])
        .unwrap();
    let added = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.5, 1., [1., 0., 0., 1.])
        .unwrap();
    for (actual, expected) in added[0][..3].iter().zip([0.3136, 0.416025, 0.5329]) {
        assert!((actual - expected).abs() < 0.000001);
    }
    assert_eq!(without[1], added[1]);
    assert_eq!(without[2], added[2]);
    recipe.layer_specular = true;
    recipe.inputs.push(input([0.4, 0.6, 0.8, 0.2]));
    let without = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.5, 0., [1., 0., 0., 1.])
        .unwrap();
    let added = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.5, 1., [1., 0., 0., 1.])
        .unwrap();
    assert_eq!(without[3], added[3]);
    assert!((added[3][3] - 0.65).abs() < 0.000001);
    let slope = |x, y| {
        input([
            (x - f32::from_bits(0xc0010204)) / f32::from_bits(0x40808102),
            (y - f32::from_bits(0xc0010204)) / f32::from_bits(0x40808102),
            1.,
            1.,
        ])
    };
    recipe.second_additive = false;
    recipe.second_normal = 2;
    recipe.layer_specular = false;
    recipe.inputs = vec![
        input([0.4, 0.5, 0.6, 0.8]),
        slope(1., 0.),
        input([0.8, 0.6, 0.4, 1.]),
        slope(0., 1.),
        input([0.2, 0.4, 0.6, 1.]),
        slope(0.5, -0.5),
    ];
    let normal = recipe
        .sample_frames(
            [0.; 2],
            [0.; 2],
            [0.; 2],
            0.5,
            0.5,
            [0., -1., 1., 0.],
            [1., 0., 0., 1.],
            [0.; 4],
        )
        .unwrap()[1];
    let length = 1.125_f32.sqrt();
    for (actual, expected) in
        normal
            .into_iter()
            .zip([0.25 / length, -0.25 / length, 1. / length, 1.])
    {
        assert!((actual - expected).abs() < 0.000001);
    }
    assert_eq!(
        (0..6)
            .map(|index| recipe.input_uv(index))
            .collect::<Vec<_>>(),
        [0, 0, 1, 1, 2, 2]
    );
    recipe.second_normal = 1;
    recipe.blend_color = true;
    recipe.inputs.remove(3);
    let only_second = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.5, 0.5, [1., 0., 0., 1.])
        .unwrap();
    let length = 1.625_f32.sqrt();
    for (actual, expected) in
        only_second[1]
            .into_iter()
            .zip([0.75 / length, -0.25 / length, 1. / length, 1.])
    {
        assert!((actual - expected).abs() < 0.000001);
    }
    assert_eq!(only_second[2][3], 0.8);
    assert_eq!(
        (0..5)
            .map(|index| recipe.input_uv(index))
            .collect::<Vec<_>>(),
        [0, 0, 1, 2, 2]
    );
}

#[test]
fn second_reveal_preserves_ordered_encoded_specular_and_independent_gloss() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: true,
        second_specular: true,
        second_parms: [0.25, 0., 0., 0.],
        second_additive: false,
        second_normal: 2,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.2, 0.4, 0.6, 0.3]),
            input([0.5, 0.5, 1., 1.]),
            input([0.6, 0.8, 0.4, 0.4]),
            input([0.5, 0.5, 1., 1.]),
            input([0.8, 0.2, 0.4, 0.5]),
            input([0.5, 0.5, 1., 1.]),
            input([0.8, 0.4, 0.6, 0.7]),
            input([0.2, 0.6, 0.4, 0.9]),
        ],
        decal: true,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: true,
        layer_specular: true,
        base_alpha_response: true,
        parms: [0.5, 0., 0., 0.],
    };
    let sample = |recipe: &Recipe, blue| {
        recipe
            .sample([0.; 2], [0.; 2], [0.; 2], 0.5, blue, [1., 0., 0., 1.])
            .unwrap()
    };
    let actual = sample(&recipe, 0.5);
    for (actual, expected) in actual[3].into_iter().zip([0.1024, 0.1936, 0.1444, 0.68]) {
        assert!((actual - expected).abs() < 0.000001);
    }
    let (base, metallic, roughness) =
        crate::material::pbr_pixel([0.3364, 0.1444, 0.2116], [0.1024, 0.1936, 0.1444], 0.68);
    for (actual, expected) in actual[0][..3].iter().zip(base) {
        assert!((actual - expected).abs() < 0.000001);
    }
    assert!((actual[2][1] - roughness).abs() < 0.000001);
    assert!((actual[2][2] - metallic).abs() < 0.000001);
    assert_eq!(
        (0..8)
            .map(|index| recipe.input_uv(index))
            .collect::<Vec<_>>(),
        [0, 0, 1, 1, 2, 2, 1, 2]
    );
    recipe.second_specular = false;
    recipe.inputs.pop();
    let low = sample(&recipe, 0.);
    let high = sample(&recipe, 1.);
    assert_eq!(low[3], high[3]);
    assert!((high[3][3] - 0.46).abs() < 0.000001);
    assert_ne!(low[0], high[0]);
    recipe.second_parms = [0.; 4];
    assert!(
        recipe
            .sample([0.; 2], [0.; 2], [0.; 2], 0.5, 0.5, [1., 0., 0., 1.])
            .is_err()
    );
    let baker = gpu::Baker::new(&recipe).unwrap();
    assert!(
        baker
            .sample(&[[
                0., 0., 0., 0., 0., 0., 0.5, 0.5, 1., 0., 0., 1., 1., 0., 0., 1., 0., 0., 0., 0.
            ]])
            .unwrap_err()
            .to_string()
            .contains("thresholds")
    );
}

#[test]
fn third_multiply_uses_vertex_alpha_and_layer_selection_uses_inclusive_half() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: true,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: true,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.2, 0.4, 0.6, 0.3]),
            input([0.5, 0.5, 1., 1.]),
            input([0.6, 0.8, 0.4, 0.4]),
            input([0.75, 0.25, 1., 1.]),
            input([0.8, 0.2, 0.4, 0.5]),
            input([0.8, 0.4, 0.6, 0.7]),
            input([0.2, 0.6, 0.4, 0.9]),
        ],
        decal: true,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: true,
        layer_specular: true,
        base_alpha_response: true,
        parms: [0.5, 0., 0., 0.],
    };
    let actual = recipe
        .sample_frames(
            [0.; 2],
            [0.; 2],
            [0.; 2],
            0.5,
            0.5,
            [1., 0., 0., 1.],
            [1., 0., 0., 1.],
            [0.2, 0.3, 0.5, 0.],
        )
        .unwrap();
    for (actual, expected) in actual[3].into_iter().zip([0.1936, 0.0784, 0.1296, 0.3358]) {
        assert!((actual - expected).abs() < 0.000001);
    }
    let (base, _, _) = crate::material::pbr_pixel(
        [0.112896, 0.238144, 0.188356],
        [0.1936, 0.0784, 0.1296],
        0.3358,
    );
    for (actual, expected) in actual[0][..3].iter().zip(base) {
        assert!((actual - expected).abs() < 0.000001);
    }
    assert_eq!(
        (0..7)
            .map(|index| recipe.input_uv(index))
            .collect::<Vec<_>>(),
        [0, 0, 1, 1, 2, 1, 3]
    );
    let without = recipe
        .sample_frames(
            [0.; 2],
            [0.; 2],
            [0.; 2],
            0.5,
            0.5,
            [1., 0., 0., 1.],
            [1., 0., 0., 1.],
            [0.2, 0.3, 0., 0.],
        )
        .unwrap();
    assert_eq!(&actual[3][..3], &without[3][..3]);
    assert!((without[3][3] - 0.46).abs() < 0.000001);
    recipe.third_multiply = false;
    recipe.layer_cutout = true;
    recipe.second_additive = false;
    recipe.decal = false;
    recipe.reveal = false;
    recipe.layer_specular = false;
    recipe.inputs.truncate(4);
    recipe.inputs[2].pixels[0][3] = 0.8;
    let low = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.62499, 1., [1., 0., 0., 1.])
        .unwrap();
    let high = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0.625, 1., [1., 0., 0., 1.])
        .unwrap();
    for (actual, expected) in low[0].into_iter().zip([0.04, 0.16, 0.36, 1.]) {
        assert!((actual - expected).abs() < 0.000001);
    }
    for (actual, expected) in high[0].into_iter().zip([0.36, 0.64, 0.16, 0.8]) {
        assert!((actual - expected).abs() < 0.000001);
    }
    assert_eq!(low[2][3], 0.3);
    assert_eq!(high[2][3], 0.3);
    assert_ne!(low[1], high[1]);
    let samples = [
        [
            0., 0., 0., 0., 0., 0., 0.62499, 1., 1., 0., 0., 1., 1., 0., 0., 1., 0., 0., 0., 0.,
        ],
        [
            0., 0., 0., 0., 0., 0., 0.625, 1., 1., 0., 0., 1., 1., 0., 0., 1., 0., 0., 0., 0.,
        ],
    ];
    let actual = gpu::Baker::new(&recipe).unwrap().sample(&samples).unwrap();
    assert_eq!(actual[0][0][3], 1.);
    assert_eq!(actual[1][0][3], 0.8);
}

#[test]
fn varying_native_frame_converts_into_solari_basis() {
    for sign in [-1., 1.] {
        let normals = [[0., 0., 1.], [0., 0.6, 0.8], [0.6, 0., 0.8]];
        let tangents = [[1., 0., 0.], [1., 0., 0.], [0.8, 0., -0.6]];
        let frame = NormalFrame {
            normals,
            tangents,
            bitangents: std::array::from_fn(|i| cross(normals[i], tangents[i]).map(|v| v * sign)),
            sign,
        };
        let sample = [0.3, -0.4, (0.75_f32).sqrt(), 1.];
        for weights in [[1., 0., 0.], [0., 1., 0.], [0.2, 0.3, 0.5]] {
            let (converted, errors) = frame.convert(weights, sample).unwrap();
            assert!(converted[2] > 0.);
            assert!(errors[0] < 0.00001);
            let interpolate = |values: [[f64; 3]; 3]| {
                std::array::from_fn(|axis| {
                    (0..3).map(|i| weights[i] * values[i][axis]).sum::<f64>()
                })
            };
            let n = unit(interpolate(normals)).unwrap();
            let t = unit(interpolate(tangents)).unwrap();
            let b = cross(n, t).map(|v| v * sign);
            let reconstructed_z =
                (1. - (converted[0] as f64).powi(2) - (converted[1] as f64).powi(2)).sqrt();
            let actual = unit(std::array::from_fn(|axis| {
                t[axis] * converted[0] as f64
                    + b[axis] * converted[1] as f64
                    + n[axis] * reconstructed_z
            }))
            .unwrap();
            let expected = unit(std::array::from_fn(|axis| {
                (0..3)
                    .map(|i| {
                        weights[i]
                            * (tangents[i][axis] * sample[0] as f64
                                + frame.bitangents[i][axis] * sample[1] as f64
                                + normals[i][axis] * sample[2] as f64)
                    })
                    .sum()
            }))
            .unwrap();
            assert!((0..3).all(|axis| (actual[axis] - expected[axis]).abs() < 0.000001));
            if weights == [0.2, 0.3, 0.5] {
                assert!(errors[1] > 0.01);
            }
        }
    }
    let singular = NormalFrame {
        normals: [[0., 0., 1.]; 3],
        tangents: [[0., 0., 1.]; 3],
        bitangents: [[0.; 3]; 3],
        sign: 1.,
    };
    assert!(singular.convert([1., 0., 0.], [0., 0., 1., 1.]).is_err());
}

#[test]
fn mixed_handedness_is_baked_into_a_consistent_output_frame() {
    let frame = NormalFrame {
        normals: [[0., 0., 1.]; 3],
        tangents: [[1., 0., 0.]; 3],
        bitangents: [[0., 1., 0.], [0., -1., 0.], [0., 1., 0.]],
        sign: 1.,
    };
    let (middle, errors) = frame.convert([0.5, 0.5, 0.], [0., 0.8, 0.6, 1.]).unwrap();
    assert_eq!(middle, [0., 0., 1., 1.]);
    assert_eq!(errors, [0.; 2]);
    let (second, errors) = frame.convert([0., 1., 0.], [0., 0.8, 0.6, 1.]).unwrap();
    assert!((second[1] + 0.8).abs() < 0.000001 && (second[2] - 0.6).abs() < 0.000001);
    assert!(errors.into_iter().all(|error| error < 0.00001));
}

#[test]
fn multiply_layer_preserves_opaque_coverage_base_normal_and_transfer_order() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.8, 0.6, 0.4, 0.]),
            input([0.75, 0.25, 1., 1.]),
            input([0.5, 0.25, 1., 0.]),
        ],
        decal: false,
        multiply: true,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: false,
        parms: [0.; 4],
    };
    let sample = |recipe: &Recipe, green| {
        recipe
            .sample([0.; 2], [0.; 2], [0.; 2], green, 1., [0., -1., 1., 0.])
            .unwrap()
    };
    let normal = sample(&recipe, 0.)[1];
    for (green, expected) in [
        (0., [0.64, 0.36, 0.16]),
        (0.5, [0.36, 0.140625, 0.16]),
        (1., [0.16, 0.0225, 0.16]),
    ] {
        let actual = sample(&recipe, green);
        for (value, expected) in actual[0][..3].iter().zip(expected) {
            assert!((value - expected).abs() < 0.000001);
        }
        assert_eq!(actual[0][3], 1.);
        assert_eq!(actual[1], normal);
    }
    let original = sample(&recipe, 0.5);
    recipe.inputs[0].pixels[0][3] = 1.;
    recipe.inputs[2].pixels[0][3] = 1.;
    assert_eq!(sample(&recipe, 0.5), original);
}

#[test]
fn second_multiply_layer_uses_blue_and_its_own_uv_before_transfer() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.8, 0.6, 0.4, 0.]),
            input([0.75, 0.25, 1., 1.]),
            input([0.5, 0.25, 1., 0.]),
            Input {
                width: 2,
                height: 1,
                sampler: 10,
                pixels: vec![[0.25, 1., 0.5, 0.], [1.; 4]],
            },
        ],
        decal: true,
        multiply: true,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: false,
        parms: [0.; 4],
    };
    let sample = |recipe: &Recipe, green, blue, uv2| {
        recipe
            .sample([0.; 2], [0.; 2], uv2, green, blue, [1., 0., 0., 1.])
            .unwrap()
    };
    let normal = sample(&recipe, 0., 0., [0.25, 0.5])[1];
    for (green, blue, expected) in [
        (0., 1., [0.04, 0.36, 0.04]),
        (1., 0., [0.16, 0.0225, 0.16]),
        (1., 1., [0.01, 0.0225, 0.04]),
        (0.5, 0.5, [0.140625, 0.140625, 0.09]),
    ] {
        let actual = sample(&recipe, green, blue, [0.25, 0.5]);
        for (value, expected) in actual[0][..3].iter().zip(expected) {
            assert!((value - expected).abs() < 0.000001);
        }
        assert_eq!(actual[0][3], 1.);
        assert_eq!(actual[1], normal);
    }
    assert_eq!(
        sample(&recipe, 1., 1., [0.75, 0.5]),
        sample(&recipe, 1., 0., [0.25, 0.5])
    );
    let original = sample(&recipe, 0.5, 0.5, [0.25, 0.5]);
    recipe.inputs[2].pixels[0][3] = 1.;
    recipe.inputs[3].pixels[0][3] = 1.;
    assert_eq!(sample(&recipe, 0.5, 0.5, [0.25, 0.5]), original);
}

#[test]
fn multiply_gloss_uses_encoded_tint_luminance_and_base_alpha() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.8, 0.6, 0.4, 0.8]),
            input([0.75, 0.25, 1., 1.]),
            input([0.5, 0.25, 1., 0.]),
        ],
        decal: false,
        multiply: true,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: true,
        parms: [0.; 4],
    };
    let sample = |recipe: &Recipe, green| {
        recipe
            .sample([0.; 2], [0.; 2], [0.; 2], green, 1., [1., 0., 0., 1.])
            .unwrap()
    };
    let mut previous_roughness = 0.;
    for (green, gloss) in [(0., 0.8), (0.5, 0.56), (1., 0.32)] {
        let actual = sample(&recipe, green);
        assert!((actual[2][3] - gloss).abs() < 0.000001);
        assert!(actual[2][1] > previous_roughness);
        assert_eq!(actual[0][3], 1.);
        assert_eq!(actual[2][2], 0.);
        previous_roughness = actual[2][1];
    }
    let original = sample(&recipe, 1.);
    recipe.inputs[2].pixels[0][3] = 1.;
    assert_eq!(sample(&recipe, 1.), original);
    recipe.inputs[0].pixels[0][3] = 0.;
    let no_gloss = sample(&recipe, 1.);
    assert_eq!(no_gloss[0], original[0]);
    assert_eq!(no_gloss[1], original[1]);
    assert_eq!(no_gloss[2][3], 0.);
    assert!((no_gloss[2][1] - 0.903602).abs() < 0.000001);
}

#[test]
fn paged_counts_preserve_distinct_model_owners_with_the_same_local_surface_index() {
    let sources = [
        ("ivy_model_a_surface0", 0, 9),
        ("ivy_model_b_surface0", 0, 6),
        ("world_surface0", 0, 3),
    ];
    let parts = [
        ("ivy_model_a_surface0", 0, 3),
        ("ivy_model_b_surface0", 0, 6),
        ("ivy_model_a_surface0", 0, 6),
        ("world_surface0", 0, 3),
    ];
    validate_paged_counts(sources, parts).unwrap();

    let mut missing = parts;
    missing[0].2 -= 3;
    assert!(validate_paged_counts(sources, missing).is_err());
    assert!(validate_paged_counts(sources, parts.into_iter().chain([parts[0]])).is_err());

    let mut reassigned = parts;
    reassigned[0].0 = "ivy_model_b_surface0";
    assert!(validate_paged_counts(sources, reassigned).is_err());
    let mut unowned = parts;
    unowned[0].1 = 1;
    assert!(validate_paged_counts(sources, unowned).is_err());
    assert!(validate_paged_counts(sources, parts.into_iter().chain([("unowned", 1, 3)])).is_err());
}

#[test]
fn oversized_chart_rotation_preserves_geometry_and_directional_metric() {
    let points = [[0., 0.], [11521., 0.], [5760., 64.]];
    let a: [f64; 2] = std::array::from_fn(|axis| points[1][axis] - points[0][axis]);
    let b: [f64; 2] = std::array::from_fn(|axis| points[2][axis] - points[0][axis]);
    let metric = [
        a[0] * a[0] + a[1] * a[1],
        a[0] * b[0] + a[1] * b[1],
        b[0] * b[0] + b[1] * b[1],
    ];
    let triangle = ChartTriangle {
        surface: 1,
        first: 12,
        points,
        metric,
        copies: vec![(1, 12, [0, 1, 2])],
    };
    let pages = place_pages(vec![Chart {
        triangles: vec![triangle],
        width: 11521,
        height: 64,
        x: 0,
        y: 0,
    }])
    .unwrap();
    assert_eq!(pages.len(), 1);
    let chart = &pages[0].charts[0];
    assert!(chart.width + 2 * GUTTER <= 8192 && chart.height + 2 * GUTTER <= 8192);
    let triangle = &chart.triangles[0];
    assert_eq!(triangle.copies, vec![(1, 12, [0, 1, 2])]);
    let a: [f64; 2] =
        std::array::from_fn(|axis| triangle.points[1][axis] - triangle.points[0][axis]);
    let b: [f64; 2] =
        std::array::from_fn(|axis| triangle.points[2][axis] - triangle.points[0][axis]);
    let rotated = [
        a[0] * a[0] + a[1] * a[1],
        a[0] * b[0] + a[1] * b[1],
        b[0] * b[0] + b[1] * b[1],
    ];
    assert!(dominates(rotated, metric) && dominates(metric, rotated));
    let impossible = ChartTriangle {
        surface: 0,
        first: 0,
        points: [[0., 0.], [20000., 0.], [0., 20000.]],
        metric: [1., 0., 1.],
        copies: vec![(0, 0, [0, 1, 2])],
    };
    assert!(
        place_pages(vec![Chart {
            triangles: vec![impossible],
            width: 20000,
            height: 20000,
            x: 0,
            y: 0
        }])
        .is_err()
    );
}

#[test]
fn additive_layer_uses_alpha_green_and_preserves_base_gloss() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.8, 0.6, 0.4, 0.75]),
            input([0.75, 0.25, 1., 1.]),
            input([0.8, 0.4, 0.2, 0.5]),
        ],
        decal: false,
        multiply: false,
        additive: true,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: true,
        parms: [0.; 4],
    };
    let sample = |recipe: &Recipe, green| {
        recipe
            .sample([0.; 2], [0.; 2], [0.; 2], green, 1., [1., 0., 0., 1.])
            .unwrap()
    };
    let original = sample(&recipe, 0.);
    for (green, expected) in [
        (0., [0.64, 0.36, 0.16]),
        (0.5, [1., 0.49, 0.2025]),
        (1., [1.44, 0.64, 0.25]),
    ] {
        let actual = sample(&recipe, green);
        for (value, expected) in actual[0][..3].iter().zip(expected) {
            assert!((value - expected).abs() < 0.000001);
        }
        assert_eq!(actual[0][3], 1.);
        assert_eq!(actual[1], original[1]);
        assert_eq!(actual[2], original[2]);
        assert_eq!(actual[2][3], 0.75);
    }
    recipe.inputs[2].pixels[0][3] = 0.;
    assert_eq!(sample(&recipe, 1.), original);
}

#[test]
fn hdr_bake_preserves_linear_color_mips_and_runtime_color_space() {
    let mut pixels = vec![[0.5, 0.25, 0., 1.]; 16];
    pixels[0] = [2.5, 0.25, 0., 1.];
    let image = mip_chain(4, 4, pixels.clone(), "color").unwrap();
    assert_eq!(image.format, 19);
    assert_eq!(texture::pixels(&image, 0).unwrap(), pixels);
    assert_eq!(texture::pixels(&image, 1).unwrap()[0], [1., 0.25, 0., 1.]);
    assert_eq!(
        texture::pixels(&image, 2).unwrap()[0],
        [0.625, 0.25, 0., 1.]
    );
    let mut pack = Pack::default();
    let native = json!({"name":"additive","technique_set":"lit_sm_r0c0n0x0_a1c1"});
    let material = store(
        &native,
        "prepared/test",
        4,
        4,
        [
            pixels,
            vec![[0., 0., 1., 1.]; 16],
            vec![[1., 0.5, 0., 0.75]; 16],
            Vec::new(),
        ],
        10,
        1,
        &mut pack,
    )
    .unwrap();
    let color = &material["extras"]["surface"]["channels"][0];
    assert_eq!(color["color_space"], "linear");
    let bytes = pack.get(color["uri"].as_str().unwrap()).unwrap();
    let metadata = texture::validate(bytes).unwrap();
    assert_eq!(metadata["vk_format"], 143);
    assert_eq!(metadata["width"], 4);
    assert_eq!(metadata["height"], 4);
    assert_eq!(metadata["levels"], 3);
    assert_eq!(bytes[u32_at(bytes, 48) as usize + 14], 1);
    assert_eq!(
        texture::encode(&image, true, "color").unwrap().as_slice(),
        bytes
    );
    for level in 0..image.levels.len() {
        assert_eq!(u64_at(bytes, 88 + level * 24), 16);
        for (channel, value) in [(3, 0.5), (0, -0.25)] {
            let mut guarded = image.clone();
            guarded.levels[level][channel * 2..channel * 2 + 2]
                .copy_from_slice(&half::f16::from_f32(value).to_le_bytes());
            let encoded = texture::encode(&guarded, true, "color").unwrap();
            assert_eq!(texture::validate(&encoded).unwrap()["vk_format"], 97);
            assert_eq!(encoded, texture::ktx2(&guarded, false).unwrap());
        }
    }
    assert!(mip_chain(0, 1, Vec::new(), "color").is_err());
    assert!(mip_chain(2, 2, vec![[1.; 4]], "color").is_err());
    for value in [f32::NAN, f32::INFINITY, -0.1, 70000.] {
        assert!(mip_chain(1, 1, vec![[value, 0., 0., 1.]], "color").is_err());
    }
    assert!(mip_chain(1, 1, vec![[2., 0., 0., 1.1]], "color").is_err());
}

#[test]
fn color_blend_uses_layer_alpha_green_and_preserves_base_normal_gloss() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.8, 0.6, 0.4, 0.75]),
            input([0.75, 0.25, 1., 1.]),
            input([0.2, 0.4, 0.8, 0.5]),
        ],
        decal: false,
        multiply: false,
        additive: false,
        blend_color: true,
        reveal: false,
        layer_specular: false,
        base_alpha_response: true,
        parms: [0.; 4],
    };
    let sample = |recipe: &Recipe, green| {
        recipe
            .sample([0.; 2], [0.; 2], [0.; 2], green, 1., [0., -1., 1., 0.])
            .unwrap()
    };
    let original = sample(&recipe, 0.);
    for (green, expected) in [
        (0., [0.64, 0.36, 0.16]),
        (0.5, [0.4225, 0.3025, 0.25]),
        (1., [0.25, 0.25, 0.36]),
    ] {
        let actual = sample(&recipe, green);
        for (value, expected) in actual[0][..3].iter().zip(expected) {
            assert!((value - expected).abs() < 0.000001);
        }
        assert_eq!(actual[0][3], 1.);
        assert_eq!(actual[1], original[1]);
        assert_eq!(actual[2], original[2]);
        assert_eq!(actual[2][3], 0.75);
    }
    recipe.inputs[2].pixels[0][3] = 0.;
    assert_eq!(sample(&recipe, 1.), original);
    recipe.inputs[2].pixels[0][3] = 1.;
    let covered = sample(&recipe, 1.);
    for (value, expected) in covered[0][..3].iter().zip([0.04, 0.16, 0.64]) {
        assert!((value - expected).abs() < 0.000001);
    }
    assert_eq!(covered[2], original[2]);
}

#[test]
fn second_color_blend_uses_blue_alpha_uv2_and_ordered_encoded_composition() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.8, 0.6, 0.4, 0.75]),
            input([0.75, 0.25, 1., 1.]),
            input([0.2, 0.4, 0.8, 0.5]),
            Input {
                width: 2,
                height: 1,
                sampler: 20,
                pixels: vec![[0.6, 0.2, 0.4, 0.5], [1., 1., 1., 0.]],
            },
        ],
        decal: true,
        multiply: false,
        additive: false,
        blend_color: true,
        reveal: false,
        layer_specular: false,
        base_alpha_response: true,
        parms: [0.; 4],
    };
    let sample = |recipe: &Recipe, green, blue, uv2| {
        recipe
            .sample([0.; 2], [0.; 2], uv2, green, blue, [1., 0., 0., 1.])
            .unwrap()
    };
    let original = sample(&recipe, 0., 0., [0.25, 0.5]);
    for (green, blue, expected) in [
        (0., 1., [0.49, 0.16, 0.16]),
        (1., 0., [0.25, 0.25, 0.36]),
        (1., 1., [0.3025, 0.1225, 0.25]),
        (0.5, 0.5, [0.40640625, 0.21390625, 0.225625]),
    ] {
        let actual = sample(&recipe, green, blue, [0.25, 0.5]);
        for (value, expected) in actual[0][..3].iter().zip(expected) {
            assert!((value - expected).abs() < 0.000001);
        }
        assert_eq!(actual[0][3], 1.);
        assert_eq!(actual[1], original[1]);
        assert_eq!(actual[2], original[2]);
    }
    assert_eq!(
        sample(&recipe, 1., 1., [0.75, 0.5]),
        sample(&recipe, 1., 0., [0.25, 0.5])
    );
    recipe.inputs[3].pixels[0][3] = 0.;
    assert_eq!(
        sample(&recipe, 1., 1., [0.25, 0.5]),
        sample(&recipe, 1., 0., [0.25, 0.5])
    );
}

#[test]
fn diffuse_reveal_zero_exponent_blends_normals_without_replacing_base_gloss() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.2, 0.4, 0.6, 0.75]),
            input([0.5, 0.5, 1., 1.]),
            input([0.8, 0.6, 0.2, 0.5]),
            input([0.75, 0.25, 1., 1.]),
        ],
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: true,
        layer_specular: false,
        base_alpha_response: true,
        parms: [0.25, 0., 0., 0.],
    };
    let sample = |green| {
        recipe
            .sample([0.; 2], [0.; 2], [0.; 2], green, 1., [1., 0., 0., 1.])
            .unwrap()
    };
    let original = sample(0.);
    for (green, expected) in [
        (0., [0.04, 0.16, 0.36]),
        (0.5, [0.25, 0.25, 0.16]),
        (1., [0.64, 0.36, 0.04]),
    ] {
        let actual = sample(green);
        for (value, expected) in actual[0][..3].iter().zip(expected) {
            assert!((value - expected).abs() < 0.000001);
        }
        assert_eq!(actual[0][3], 1.);
        assert_eq!(actual[2], original[2]);
        assert_eq!(actual[2][3], 0.75);
        assert!((actual[1][..3].iter().map(|v| v * v).sum::<f32>() - 1.).abs() < 0.000001);
    }
    assert!(sample(1.)[1][0] > sample(0.)[1][0]);
    assert!(sample(1.)[1][1] < sample(0.)[1][1]);
}

#[test]
fn base_alpha_reveal_preserves_mixed_native_gloss_and_zero_exponent() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.2, 0.4, 0.6, 0.75]),
            input([0.5, 0.5, 1., 1.]),
            input([0.8, 0.6, 0.2, 0.5]),
            input([0.5, 0.5, 1., 1.]),
            input([0.6, 0.4, 0.2, 0.25]),
        ],
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: true,
        layer_specular: true,
        base_alpha_response: true,
        parms: [0.5, 0.5, 0., 0.],
    };
    for (green, expected) in [
        (0., [0.04, 0.04, 0.04, 0.75]),
        (0.5, [0.16, 0.09, 0.04, 0.5]),
        (1., [0.36, 0.16, 0.04, 0.25]),
    ] {
        let actual = recipe
            .sample([0.; 2], [0.; 2], [0.; 2], green, 1., [1., 0., 0., 1.])
            .unwrap();
        for (value, expected) in actual[3].iter().zip(expected) {
            assert!((value - expected).abs() < 0.000001);
        }
        assert_eq!(actual[0][3], 1.);
    }
    recipe.parms = [1., 0., 0., 0.];
    let reference = recipe
        .sample([0.; 2], [0.; 2], [0.; 2], 0., 1., [1., 0., 0., 1.])
        .unwrap();
    for green in [0., 0.5, 1.] {
        assert_eq!(
            recipe
                .sample([0.; 2], [0.; 2], [0.; 2], green, 1., [1., 0., 0., 1.])
                .unwrap(),
            reference
        );
    }
    assert!((reference[3][3] - 0.5).abs() < 0.000001);
}

#[test]
fn layered_base_alpha_response_survives_complete_layer_coverage() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.2, 0.4, 0.6, 0.]),
            input([0.5, 0.5, 1., 1.]),
            input([0.8, 0.2, 0.4, 1.]),
            input([0.5, 0.5, 1., 1.]),
        ],
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: true,
        parms: [0.; 4],
    };
    for (alpha, roughness) in [(0., 0.903602), (1., 0.12499237)] {
        recipe.inputs[0].pixels[0][3] = alpha;
        for green in [0., 0.5, 1.] {
            let actual = recipe
                .sample([0.; 2], [0.; 2], [0.; 2], green, 1., [1., 0., 0., 1.])
                .unwrap();
            assert!((actual[2][1] - roughness).abs() < 0.000001);
            assert_eq!(actual[2][2], 0.);
            assert_eq!(actual[2][3], alpha);
            assert_eq!(actual[0][3], 1.);
            if green == 1. {
                for (value, expected) in actual[0][..3].iter().zip([0.64, 0.04, 0.16]) {
                    assert!((value - expected).abs() < 0.000001);
                }
            }
        }
    }
}

#[test]
fn reveal_decal_preserves_uv2_axis_addressing_and_encoded_composition() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.2, 0.4, 0.6, 0.]),
            input([0.5, 0.5, 1., 1.]),
            input([0.8, 0.2, 0.4, 0.5]),
            input([0.75, 0.25, 1., 1.]),
            Input {
                width: 2,
                height: 2,
                sampler: 74,
                pixels: vec![
                    [0.9, 0.8, 0.7, 1.],
                    [0.1, 0.2, 0.3, 1.],
                    [0.5, 0.25, 0.75, 0.],
                    [1., 0.5, 0.25, 0.],
                ],
            },
        ],
        decal: true,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: true,
        layer_specular: false,
        base_alpha_response: false,
        parms: [0.0713300034403801, 0.5, 0., 0.],
    };
    let sample = |recipe: &Recipe, blue| {
        recipe
            .sample([0.; 2], [0.; 2], [1.25, 1.25], 0.5, blue, [0., -1., 1., 0.])
            .unwrap()
    };
    let actual = sample(&recipe, 0.5);
    for (value, expected) in actual[0][..3]
        .iter()
        .zip([0.140625, 0.03515625, 0.19140625])
    {
        assert!((value - expected).abs() < 0.000001);
    }
    assert_eq!(actual[1], sample(&recipe, 0.)[1]);
    assert_eq!(actual[2], [1., 1., 0., 1.]);
    assert_eq!(actual[3], [0.; 4]);
    for pixel in &mut recipe.inputs[4].pixels {
        pixel[3] = 1. - pixel[3];
    }
    assert_eq!(actual, sample(&recipe, 0.5));
    recipe.inputs[4].sampler = 10;
    assert_ne!(actual[0], sample(&recipe, 0.5)[0]);
}

#[test]
fn diffuse_reveal_uses_four_inputs_and_green_only() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let recipe = Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: false,
        geometric_normal: false,
        inputs: vec![
            input([0.2, 0.4, 0.6, 0.]),
            input([0.5, 0.5, 1., 1.]),
            input([0.8, 0.6, 0.2, 0.5]),
            input([0.5, 0.5, 1., 1.]),
        ],
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: true,
        layer_specular: false,
        base_alpha_response: false,
        parms: [0.0713300034403801, 0.5, 0., 0.],
    };
    for (green, expected) in [
        (0., [0.04, 0.16, 0.36]),
        (0.5, [0.25, 0.25, 0.16]),
        (1., [0.64, 0.36, 0.04]),
    ] {
        let low = recipe
            .sample([0.; 2], [0.; 2], [0.; 2], green, 0., [1., 0., 0., 1.])
            .unwrap();
        let high = recipe
            .sample([0.; 2], [0.; 2], [0.; 2], green, 1., [1., 0., 0., 1.])
            .unwrap();
        for (actual, expected) in low[0][..3].iter().copied().zip(expected) {
            assert!((actual - expected).abs() < 0.000001);
        }
        assert_eq!(low, high);
        assert_eq!(low[2], [1., 1., 0., 1.]);
        assert_eq!(low[3], [0.; 4]);
    }
}

#[test]
fn alpha_reveal_gpu_uses_interpolated_alpha_without_changing_rgb_or_specular() {
    let input = |pixel| Input {
        width: 1,
        height: 1,
        sampler: 20,
        pixels: vec![pixel],
    };
    let mut recipe = Recipe {
        alpha_reveal: true,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply: false,
        layer_cutout: false,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        second_additive: false,
        second_normal: 0,
        base_specular: true,
        geometric_normal: false,
        inputs: vec![
            input([0.6, 0.5, 0.4, 0.6]),
            input([0.5, 0.5, 1., 1.]),
            input([0.2, 0.3, 0.4, 1.]),
        ],
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: false,
        parms: [0.25, 0., 0., 0.],
    };

    for specular in [false, true] {
        recipe.base_specular = specular;
        for exponent in [0., 0.5] {
            recipe.parms[1] = exponent;
            let samples: Vec<gpu::Sample> = [0., 0.125, 0.25, 0.5, 0.75, 0.875, 1.]
                .into_iter()
                .map(|alpha| {
                    let mut sample = [0.; 20];
                    sample[6] = 0.91;
                    sample[7] = 0.83;
                    sample[18] = alpha;
                    sample
                })
                .collect();
            let actual = gpu::Baker::new(&recipe).unwrap().sample(&samples).unwrap();
            for (sample, actual) in samples.iter().zip(&actual) {
                let expected = recipe
                    .sample_frames(
                        [0.; 2],
                        [0.; 2],
                        [0.; 2],
                        sample[6],
                        sample[7],
                        [0.; 4],
                        [0.; 4],
                        [0., 0., sample[18], 0.],
                    )
                    .unwrap();
                for (actual, expected) in actual.iter().flatten().zip(expected.iter().flatten()) {
                    assert!(
                        (actual - expected).abs() < 0.00003,
                        "GPU {actual}, CPU {expected}"
                    );
                }
            }
            assert!(actual[0][0][3] < 0.001);
            let threshold_width = 0.25 * 0.5_f32.powf(exponent);
            let expected_midpoint = (0.6 - (0.5 - threshold_width)) / (2. * threshold_width);
            assert!((actual[3][0][3] - expected_midpoint).abs() < 0.00001);
            assert!(actual[6][0][3] > 0.999);
            for pixel in &actual[1..] {
                assert_eq!(pixel[0][..3], actual[0][0][..3]);
                assert_eq!(pixel[1], actual[0][1]);
                assert_eq!(pixel[2], actual[0][2]);
                assert_eq!(pixel[3], actual[0][3]);
            }
        }
    }
}
